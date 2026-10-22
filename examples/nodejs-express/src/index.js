import express from "express";

const app = express();
const port = Number(process.env.PORT || 3000);

app.get("/", (_request, response) => {
  response.status(200).type("html").send("<!doctype html><html><body><h1>Hello World</h1></body></html>");
});

app.get("/healthz", (_request, response) => {
  response.sendStatus(200);
});

const server = app.listen(port, "0.0.0.0");

for (const signal of ["SIGTERM", "SIGINT"]) {
  process.on(signal, () => {
    server.close((error) => {
      if (error) {
        console.error(error);
        process.exitCode = 1;
      }
    });
  });
}