use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode, Uri},
    response::IntoResponse,
};

pub async fn serve_static(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    
    // SECURITY: Prevent path traversal attacks
    if path.contains("..") || path.contains("//") || path.contains("\\") {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Invalid path"))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal server error"))
                    .unwrap()
            });
    }
    
    // Serve UI - in production, this would read from dist/ directory
    // For development, redirect to the Vite dev server
    // SECURITY: Only handle non-API routes - API routes should be handled by the router
    // If we reach this fallback handler, it means no route matched, so we should only
    // serve static files for non-API paths
    if path.is_empty() || (!path.starts_with("api/") && !path.starts_with("metrics") && !path.starts_with("health")) {
        // Return a simple HTML page that redirects to the UI dev server
        // In production, this would serve the actual built files
        let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>NarayanaDB UI</title>
    <meta http-equiv="refresh" content="0; url=http://localhost:3000">
    <script>
        window.location.href = 'http://localhost:3000';
    </script>
</head>
<body>
    <p>Redirecting to NarayanaDB UI... <a href="http://localhost:3000">Click here</a></p>
</body>
</html>
        "#;
        
        // SECURITY: Handle builder errors gracefully instead of unwrap
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html")
            .body(Body::from(html))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal server error"))
                    .unwrap()
            })
    } else {
        // API routes that don't match should return 404
        // This should not happen if routes are properly registered
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal server error"))
                    .unwrap()
            })
    }
}

