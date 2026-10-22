package example.springboot;

import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
public class HelloController {
    @GetMapping(value = "/", produces = MediaType.TEXT_HTML_VALUE)
    public String helloWorld() {
        return "<!doctype html><html><body><h1>Hello World</h1></body></html>";
    }

    @GetMapping("/healthz")
    public ResponseEntity<Void> health() {
        return ResponseEntity.ok().build();
    }
}