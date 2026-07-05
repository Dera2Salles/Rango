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
        if !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                let key: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                let val: LitStr = input.parse()?;
                if key == "table" {
                    table = Some(val.value());
                }
            }
        }
        Ok(ModelAttr { table })
    }
}

pub fn expand_model(attr: ModelAttr, input_struct: ItemStruct) -> TokenStream2 {
    let struct_name = &input_struct.ident;
    let table_name = attr.table.unwrap_or_else(|| {
        let name = struct_name.to_string().to_lowercase();
        if name.ends_y() {
            format!("{}ies", &name[..name.len() - 1])
        } else if name.ends_s() {
            format!("{}es", name)
        } else {
            format!("{}s", name)
        }
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

    // Nom du champ id en string (pour interpolation SQL)
    let id_field_name = id_field.to_string();

    let mut fields_names: Vec<String> = Vec::new();
    let mut fields_vars: Vec<TokenStream2> = Vec::new();

    let mut admin_fields = Vec::new();
    let mut from_form_fields = Vec::new();
    let mut update_form_fields = Vec::new();

    if let syn::Fields::Named(fields) = &input_struct.fields {
        for field in &fields.named {
            if let Some(ident) = &field.ident {
                let name = ident.to_string();
                let ty = &field.ty;
                let ty_str = quote! { #ty }.to_string().replace(" ", "");

                let is_id = name == id_field_name;
                let editable = !is_id;

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
                } else {
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
                    } else if ty_str == "i64" || ty_str == "i32" || ty_str == "u64" || ty_str == "u32" {
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
                    } else if ty_str.contains("Option") {
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

                    // Seulement les champs non-id vont dans INSERT/UPDATE
                    fields_names.push(name.clone());
                    fields_vars.push(quote! { self.#ident });
                }
            }
        }
    }

    let fields_names_lits: Vec<String> = fields_names.clone();

    quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
        #input_struct

        #[::rango::axum::async_trait]
        impl ::rango::db::RangoModel for #struct_name {
            fn table_name() -> &'static str {
                #table_name
            }

            async fn save(&mut self) -> ::rango::RangoResult<()> {
                use ::rango::db::{backend, placeholder, db};
                use ::rango::state::DatabaseBackend;

                let field_names: &[&str] = &[#(#fields_names_lits),*];
                let backend_ref = backend()?;
                let pool = db()?;

                if self.#id_field == 0 {
                    // ─── INSERT ───
                    let cols = field_names.join(", ");
                    let placeholders: Vec<String> = (1..=field_names.len())
                        .map(|i| placeholder(backend_ref, i))
                        .collect();
                    let placeholders_str = placeholders.join(", ");

                    match backend_ref {
                        DatabaseBackend::Postgres => {
                            // Postgres : RETURNING id pour récupérer le nouvel id
                            let q = format!(
                                "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
                                #table_name, cols, placeholders_str, #id_field_name
                            );
                            let row: (i64,) = ::rango::sqlx::query_as(&q)
                                #(.bind(&#fields_vars))*
                                .fetch_one(pool)
                                .await
                                .map_err(|e| ::rango::error::RangoError::DatabaseError(e.to_string()))?;
                            self.#id_field = row.0;
                        }
                        _ => {
                            // SQLite / MySQL : last_insert_id()
                            let q = format!(
                                "INSERT INTO {} ({}) VALUES ({})",
                                #table_name, cols, placeholders_str
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
                    // ─── UPDATE ───
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
                        #table_name, sets.join(", "), #id_field_name, id_ph
                    );
                    ::rango::sqlx::query(&q)
                        #(.bind(&#fields_vars))*
                        .bind(self.#id_field)
                        .execute(pool)
                        .await
                        .map_err(|e| ::rango::error::RangoError::DatabaseError(e.to_string()))?;
                }
                Ok(())
            }

            async fn delete(&self) -> ::rango::RangoResult<u64> {
                use ::rango::db::{backend, placeholder, db};
                let backend_ref = backend()?;
                let pool = db()?;
                let ph = placeholder(backend_ref, 1);
                let q = format!(
                    "DELETE FROM {} WHERE {} = {}",
                    #table_name, #id_field_name, ph
                );
                let result = ::rango::sqlx::query(&q)
                    .bind(self.#id_field)
                    .execute(pool)
                    .await
                    .map_err(|e| ::rango::error::RangoError::DatabaseError(e.to_string()))?;
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
    }
}

trait StrExt {
    fn ends_y(&self) -> bool;
    fn ends_s(&self) -> bool;
}

impl StrExt for String {
    fn ends_y(&self) -> bool {
        self.ends_with('y')
    }
    fn ends_s(&self) -> bool {
        self.ends_with('s')
    }
}
