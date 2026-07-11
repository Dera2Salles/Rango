use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    ItemStruct, LitStr, Result, Token,
};

pub struct ModelAttr {
    pub table: Option<String>,
}

impl Parse for ModelAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut table = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "table" {
                let val: LitStr = input.parse()?;
                table = Some(val.value());
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(ModelAttr { table })
    }
}

pub fn expand_model(attr: ModelAttr, input_struct: ItemStruct) -> TokenStream2 {
    let struct_name = &input_struct.ident;
    let table_name = attr.table.unwrap_or_else(|| {
        let name = struct_name.to_string().to_lowercase();
        pluralize(&name)
    });

    let mut id_field = quote! { id };

    // Inspect fields for #[rango(id)] or default to "id"
    if let syn::Fields::Named(fields) = &input_struct.fields {
        for field in &fields.named {
            for attr in &field.attrs {
                if attr.path().is_ident("rango") {
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("id") {
                            if let Some(ident) = &field.ident {
                                id_field = quote! { #ident };
                            }
                        }
                        Ok(())
                    });
                }
            }
        }
    }

    let id_field_name = id_field.to_string();

    let mut fields_names: Vec<String> = Vec::new();
    let mut fields_vars: Vec<TokenStream2> = Vec::new();
    let mut column_defs: Vec<TokenStream2> = Vec::new();
    let mut index_defs: Vec<TokenStream2> = Vec::new();

    let mut admin_fields = Vec::new();
    let mut from_form_fields = Vec::new();
    let mut update_form_fields = Vec::new();

    if let syn::Fields::Named(fields) = &input_struct.fields {
        for field in &fields.named {
            if let Some(ident) = &field.ident {
                let name = ident.to_string();
                let ty = &field.ty;
                let ty_str = quote! { #ty }.to_string().replace(' ', "");

                let is_id = name == id_field_name;
                let editable = !is_id;

                let mut is_unique = false;
                let mut is_nullable = false;
                let mut is_indexed = false;
                let mut has_default: Option<String> = None;

                for a in &field.attrs {
                    if a.path().is_ident("rango") {
                        let _ = a.parse_nested_meta(|meta| {
                            if meta.path.is_ident("unique") {
                                is_unique = true;
                            } else if meta.path.is_ident("nullable") {
                                is_nullable = true;
                            } else if meta.path.is_ident("index") {
                                is_indexed = true;
                            } else if meta.path.is_ident("default") {
                                let val: LitStr = meta.value()?.parse()?;
                                has_default = Some(val.value());
                            }
                            Ok(())
                        });
                    }
                }

                let sql_type = rust_type_to_sql(&ty_str);

                admin_fields.push(quote! {
                    ::rango::db::AdminField {
                        name: #name.to_string(),
                        field_type: #ty_str.to_string(),
                        editable: #editable,
                    }
                });

                if is_id {
                    from_form_fields.push(quote! {
                        #ident: form_data.get(#name).and_then(|v| v.parse().ok()).unwrap_or(0)
                    });

                    column_defs.push(quote! {
                        ::rango::db::ColumnDef::new(#name, "INTEGER")
                            .primary_key()
                    });
                } else {
                    let default_val = has_default.clone().unwrap_or_default();
                    let has_default_val = has_default.is_some();
                    column_defs.push(quote! {
                        {
                            let mut col = ::rango::db::ColumnDef::new(#name, #sql_type);
                            if #is_nullable { col = col.nullable(); }
                            if #is_unique { col = col.unique(); }
                            if #has_default_val { col = col.default(#default_val); }
                            col
                        }
                    });

                    if is_indexed || is_unique {
                        index_defs.push(quote! {
                            format!(
                                "CREATE INDEX IF NOT EXISTS idx_{}_{} ON {} ({});",
                                #table_name, #name, #table_name, #name
                            )
                        });
                    }

                    if ty_str == "String" {
                        from_form_fields.push(quote! {
                            #ident: form_data.get(#name).cloned().unwrap_or_default()
                        });
                        update_form_fields.push(quote! {
                            if let Some(val) = form_data.get(#name) {
                                self.#ident = val.clone();
                            }
                        });
                    } else if ty_str == "bool" {
                        from_form_fields.push(quote! {
                            #ident: form_data.get(#name)
                                .map(|v| v == "true" || v == "on" || v == "1")
                                .unwrap_or(false)
                        });
                        update_form_fields.push(quote! {
                            self.#ident = form_data.get(#name)
                                .map(|v| v == "true" || v == "on" || v == "1")
                                .unwrap_or(false);
                        });
                    } else if ty_str == "i64"
                        || ty_str == "i32"
                        || ty_str == "u64"
                        || ty_str == "u32"
                        || ty_str == "i16"
                        || ty_str == "u16"
                        || ty_str == "i8"
                        || ty_str == "u8"
                    {
                        from_form_fields.push(quote! {
                            #ident: form_data.get(#name).and_then(|v| v.parse().ok()).unwrap_or(0)
                        });
                        update_form_fields.push(quote! {
                            if let Some(val) = form_data.get(#name).and_then(|v| v.parse().ok()) {
                                self.#ident = val;
                            }
                        });
                    } else if ty_str == "f64" || ty_str == "f32" {
                        from_form_fields.push(quote! {
                            #ident: form_data.get(#name).and_then(|v| v.parse().ok()).unwrap_or(0.0)
                        });
                        update_form_fields.push(quote! {
                            if let Some(val) = form_data.get(#name).and_then(|v| v.parse().ok()) {
                                self.#ident = val;
                            }
                        });
                    } else if ty_str.starts_with("Option") {
                        from_form_fields.push(quote! {
                            #ident: form_data.get(#name).filter(|s| !s.is_empty()).cloned()
                        });
                        update_form_fields.push(quote! {
                            self.#ident = form_data.get(#name).filter(|s| !s.is_empty()).cloned();
                        });
                    } else {
                        from_form_fields.push(quote! {
                            #ident: Default::default()
                        });
                    }

                    fields_names.push(name.clone());
                    fields_vars.push(quote! { self.#ident });
                }
            }
        }
    }

    let fields_names_lits: Vec<String> = fields_names.clone();
    let table_name_lit = table_name.clone();

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
        #input_struct

        #[::rango::axum::async_trait]
        impl ::rango::db::RangoModel for #struct_name {
            fn table_name() -> &'static str {
                #table_name_lit
            }

            fn id_column() -> &'static str {
                #id_field_name
            }

            fn pk(&self) -> i64 {
                self.#id_field
            }

            fn set_pk(&mut self, id: i64) {
                self.#id_field = id;
            }

            async fn save(&mut self) -> ::rango::RangoResult<()> {
                use ::rango::db::{backend, placeholder, db};
                use ::rango::state::DatabaseBackend;

                let serialized = ::rango::serde_json::to_value(&self).unwrap_or(::rango::serde_json::Value::Null);
                ::rango::signals::PRE_SAVE.send(&serialized);

                let field_names: &[&str] = &[#(#fields_names_lits),*];
                let backend_ref = backend()?;
                let pool = db()?;

                if self.#id_field == 0 {
                    let cols = field_names.join(", ");
                    let placeholders: Vec<String> = (1..=field_names.len())
                        .map(|i| placeholder(backend_ref, i))
                        .collect();
                    let placeholders_str = placeholders.join(", ");

                    match backend_ref {
                        DatabaseBackend::Postgres => {
                            let q = format!(
                                "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
                                #table_name_lit, cols, placeholders_str, #id_field_name
                            );
                            let row: (i64,) = ::rango::sqlx::query_as(&q)
                                #(.bind(&#fields_vars))*
                                .fetch_one(pool)
                                .await
                                .map_err(|e| ::rango::error::RangoError::DatabaseError(e.to_string()))?;
                            self.#id_field = row.0;
                        }
                        _ => {
                            let q = format!(
                                "INSERT INTO {} ({}) VALUES ({})",
                                #table_name_lit, cols, placeholders_str
                            );
                            let result = ::rango::sqlx::query(&q)
                                #(.bind(&#fields_vars))*
                                .execute(pool)
                                .await
                                .map_err(|e| ::rango::error::RangoError::DatabaseError(e.to_string()))?;
                            if let Some(new_id) = result.last_insert_id() {
                                self.#id_field = new_id as i64;
                            }
                        }
                    }
                } else {
                    let sets: Vec<String> = field_names
                        .iter()
                        .enumerate()
                        .map(|(i, col)| {
                            format!("{} = {}", col, placeholder(backend_ref, i + 1))
                        })
                        .collect();
                    let id_ph = placeholder(backend_ref, field_names.len() + 1);
                    let q = format!(
                        "UPDATE {} SET {} WHERE {} = {}",
                        #table_name_lit, sets.join(", "), #id_field_name, id_ph
                    );
                    ::rango::sqlx::query(&q)
                        #(.bind(&#fields_vars))*
                        .bind(self.#id_field)
                        .execute(pool)
                        .await
                        .map_err(|e| ::rango::error::RangoError::DatabaseError(e.to_string()))?;
                }

                let final_serialized = ::rango::serde_json::to_value(&self).unwrap_or(::rango::serde_json::Value::Null);
                ::rango::signals::POST_SAVE.send(&final_serialized);
                Ok(())
            }

            async fn delete(&self) -> ::rango::RangoResult<u64> {
                use ::rango::db::{backend, placeholder, db};

                let serialized = ::rango::serde_json::to_value(&self).unwrap_or(::rango::serde_json::Value::Null);
                ::rango::signals::PRE_DELETE.send(&serialized);

                let backend_ref = backend()?;
                let pool = db()?;
                let ph = placeholder(backend_ref, 1);
                let q = format!(
                    "DELETE FROM {} WHERE {} = {}",
                    #table_name_lit, #id_field_name, ph
                );
                let result = ::rango::sqlx::query(&q)
                    .bind(self.#id_field)
                    .execute(pool)
                    .await
                    .map_err(|e| ::rango::error::RangoError::DatabaseError(e.to_string()))?;

                ::rango::signals::POST_DELETE.send(&serialized);
                Ok(result.rows_affected())
            }
        }

        impl ::rango::db::RangoAdminMetadata for #struct_name {
            fn model_name() -> &'static str {
                stringify!(#struct_name)
            }

            fn fields() -> Vec<::rango::db::AdminField> {
                vec![#(#admin_fields),*]
            }

            fn to_json_value(&self) -> ::rango::serde_json::Value {
                ::rango::serde_json::to_value(self)
                    .unwrap_or(::rango::serde_json::Value::Null)
            }

            fn from_form(
                form_data: &std::collections::HashMap<String, String>,
            ) -> Result<Self, String> {
                Ok(#struct_name {
                    #(#from_form_fields),*
                })
            }

            fn update_from_form(
                &mut self,
                form_data: &std::collections::HashMap<String, String>,
            ) -> Result<(), String> {
                #(#update_form_fields)*
                Ok(())
            }
        }

        impl ::rango::db::RangoSchema for #struct_name {
            fn columns() -> Vec<::rango::db::ColumnDef> {
                vec![#(#column_defs),*]
            }

            fn generate_migration_sql() -> String {
                let cols: Vec<String> = Self::columns().iter().map(|c| c.to_sql()).collect();
                format!(
                    "CREATE TABLE IF NOT EXISTS {} (\n    {}\n);",
                    #table_name_lit,
                    cols.join(",\n    ")
                )
            }

            fn generate_index_sql() -> Vec<String> {
                vec![#(#index_defs),*]
            }
        }
    }
}

/// Rust type → SQL type mapping (multi-DB aware).
fn rust_type_to_sql(ty: &str) -> &'static str {
    match ty {
        "String" | "&str" => "TEXT",
        "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "usize" | "isize" => {
            "INTEGER"
        }
        "f64" | "f32" => "REAL",
        "bool" => "BOOLEAN",
        t if t.contains("Option<String>") => "TEXT",
        t if t.contains("Option<i") || t.contains("Option<u") => "INTEGER",
        t if t.contains("Option<f") => "REAL",
        t if t.contains("Option<bool>") => "BOOLEAN",
        t if t.contains("Vec<u8>") => "BLOB",
        t if t.contains("Uuid") || t.contains("uuid::Uuid") => "TEXT",
        t if t.contains("NaiveDateTime") || t.contains("DateTime") || t.contains("chrono::") => {
            "TIMESTAMP"
        }
        _ => "TEXT",
    }
}

/// Pluralize a lowercase model name.
fn pluralize(name: &str) -> String {
    if name.ends_with('y') && !name.ends_with("ay") && !name.ends_with("ey") {
        format!("{}ies", &name[..name.len() - 1])
    } else if name.ends_with('s')
        || name.ends_with('x')
        || name.ends_with('z')
        || name.ends_with("ch")
        || name.ends_with("sh")
    {
        format!("{}es", name)
    } else {
        format!("{}s", name)
    }
}
