var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.MapGet("/", () => Results.Content(
    "<!doctype html><html><body><h1>Hello World</h1></body></html>",
    "text/html; charset=utf-8"));
app.MapGet("/healthz", () => Results.Ok(new { status = "ok" }));

app.Run();