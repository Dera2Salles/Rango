use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

pub const DEFAULT_404_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Page Not Found | Rango</title>
    <style>
        :root {
            --primary: #e94560;
            --bg: #0f172a;
            --text: #f8fafc;
            --muted: #94a3b8;
        }
        body {
            background-color: var(--bg);
            color: var(--text);
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
            text-align: center;
        }
        .container {
            max-width: 600px;
            padding: 2rem;
        }
        h1 {
            font-size: 8rem;
            margin: 0;
            color: var(--primary);
            line-height: 1;
            text-shadow: 4px 4px 0px rgba(233, 69, 96, 0.2);
        }
        h2 {
            font-size: 2rem;
            margin: 1rem 0;
        }
        p {
            color: var(--muted);
            font-size: 1.1rem;
            margin-bottom: 2rem;
        }
        .btn {
            background-color: var(--primary);
            color: white;
            text-decoration: none;
            padding: 0.75rem 1.5rem;
            border-radius: 0.5rem;
            font-weight: bold;
            transition: transform 0.2s, background-color 0.2s;
        }
        .btn:hover {
            background-color: #d13d55;
            transform: translateY(-2px);
        }
        .icon {
            font-size: 4rem;
            margin-bottom: 1rem;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="icon">🤠</div>
        <h1>404</h1>
        <h2>Oops! This trail is a dead end.</h2>
        <p>The page you are looking for seems to have vanished into the Rango desert.</p>
        <a href="/" class="btn">Back to Home</a>
    </div>
</body>
</html>
"#;

pub async fn default_404_handler() -> impl IntoResponse {
    #[cfg(feature = "templates")]
    {
        // Try to render 404.html if it exists
        if let Ok(response) = crate::responses::render("404.html", serde_json::json!({})) {
            return (StatusCode::NOT_FOUND, response).into_response();
        }
    }

    (StatusCode::NOT_FOUND, Html(DEFAULT_404_HTML)).into_response()
}
