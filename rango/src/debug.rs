use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use crate::state::config;

pub async fn debug_error_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    let response = next.run(req).await;

    if response.status().is_server_error() && config().debug {
        // Here we could ideally capture the error from the response extensions
        // but since we want to show a nice page for 500s in general when debug is on:
        return render_debug_page(
            response.status(),
            "An internal server error occurred.",
            &method.to_string(),
            &uri.to_string(),
            &format!("{:?}", headers),
        ).into_response();
    }

    response
}

pub fn render_debug_page(
    status: StatusCode,
    message: &str,
    method: &str,
    uri: &str,
    headers: &str,
) -> Html<String> {
    let html = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🤠 Rango Debugger - {status}</title>
    <style>
        :root {{
            --bg-main: #0f172a;
            --bg-card: #1e293b;
            --primary: #e94560;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --accent: #38bdf8;
            --code-bg: #000000;
        }}
        body {{
            background-color: var(--bg-main);
            color: var(--text-main);
            font-family: 'Inter', ui-sans-serif, system-ui, sans-serif;
            margin: 0;
            line-height: 1.5;
        }}
        header {{
            background-color: var(--primary);
            padding: 2rem;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
        }}
        .container {{
            max-width: 1200px;
            margin: 2rem auto;
            padding: 0 1rem;
        }}
        .error-card {{
            background-color: var(--bg-card);
            border-radius: 0.75rem;
            padding: 2rem;
            margin-bottom: 2rem;
            border-left: 6px solid var(--primary);
        }}
        .error-title {{
            font-size: 1.5rem;
            font-weight: 700;
            margin-bottom: 0.5rem;
            color: var(--primary);
        }}
        .error-message {{
            font-size: 2rem;
            font-weight: 800;
            margin-bottom: 1rem;
        }}
        .info-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 1.5rem;
        }}
        .info-card {{
            background-color: var(--bg-card);
            border-radius: 0.5rem;
            padding: 1.5rem;
        }}
        .info-card h3 {{
            margin-top: 0;
            color: var(--accent);
            border-bottom: 1px solid #334155;
            padding-bottom: 0.5rem;
            font-size: 1.1rem;
        }}
        pre {{
            background-color: var(--code-bg);
            padding: 1rem;
            border-radius: 0.4rem;
            overflow-x: auto;
            font-size: 0.9rem;
            color: #d1d5db;
        }}
        .badge {{
            display: inline-block;
            padding: 0.25rem 0.5rem;
            background: var(--primary);
            border-radius: 0.25rem;
            font-size: 0.8rem;
            font-weight: bold;
            margin-right: 0.5rem;
        }}
        .method-badge {{
            background: var(--accent);
            color: var(--bg-main);
        }}
    </style>
</head>
<body>
    <header>
        <div style="max-width: 1200px; margin: 0 auto; display: flex; align-items: center;">
            <span style="font-size: 2.5rem; margin-right: 1rem;">🤠</span>
            <div>
                <h1 style="margin: 0; font-size: 1.5rem;">Rango Debugger</h1>
                <p style="margin: 0; opacity: 0.9;">Something went wrong on the trail.</p>
            </div>
        </div>
    </header>

    <div class="container">
        <div class="error-card">
            <div class="error-title">Unhandled Exception</div>
            <div class="error-message">{message}</div>
            <div>
                <span class="badge">{status}</span>
                <span class="badge method-badge">{method}</span>
                <code style="color: var(--accent);">{uri}</code>
            </div>
        </div>

        <div class="info-grid">
            <div class="info-card">
                <h3>Request Details</h3>
                <p><strong>Method:</strong> {method}</p>
                <p><strong>URL:</strong> {uri}</p>
            </div>
            <div class="info-card">
                <h3>Headers</h3>
                <pre>{headers}</pre>
            </div>
            <div class="info-card">
                <h3>Environment</h3>
                <p><strong>Rango Version:</strong> 0.1.0</p>
                <p><strong>Debug Mode:</strong> Enabled</p>
                <p><strong>OS:</strong> {os}</p>
            </div>
        </div>

        <div class="info-card" style="margin-top: 2rem;">
            <h3>Possible Solutions</h3>
            <ul style="color: var(--text-muted);">
                <li>Check your database connection if you're using a model.</li>
                <li>Verify that the template exists in the <code>templates/</code> directory.</li>
                <li>Ensure all environment variables are correctly set.</li>
            </ul>
        </div>
    </div>
</body>
</html>
"#,
        status = status,
        message = message,
        method = method,
        uri = uri,
        headers = headers,
        os = std::env::consts::OS,
    );
    Html(html)
}
