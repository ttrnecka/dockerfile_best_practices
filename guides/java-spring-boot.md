# Java Web API with Spring Boot

## Target and Application Shape

Checked 2026-10-21. This example targets Java 25 and Spring Boot 4.1.1. Spring Boot 4.1.1 requires Java 17 or newer and supports Java through 26; Java 25 is an OpenJDK LTS release. The example assumes a Maven project-root context with the Maven Wrapper configured for Maven 3.10.0, `.mvn/wrapper/`, `pom.xml`, and `src/`. The POM sets the artifact's final name to `application.jar`, targets Java 25, and includes a Spring Boot executable JAR.

## Dockerfile

```dockerfile
FROM docker.io/library/eclipse-temurin:25-jdk-noble AS build

WORKDIR /workspace
COPY mvnw pom.xml ./
COPY .mvn/ ./.mvn/
RUN chmod +x mvnw && ./mvnw -B -ntp dependency:go-offline

COPY src/ ./src/
RUN ./mvnw -B -ntp verify \
    && mkdir -p /out \
    && cp target/application.jar /out/app.jar \
    && java -Djarmode=tools -jar /out/app.jar extract --layers --destination /out/extracted

FROM docker.io/library/eclipse-temurin:25-jre-noble

WORKDIR /app
COPY --from=build /out/extracted/dependencies/ ./
COPY --from=build /out/extracted/spring-boot-loader/ ./
COPY --from=build /out/extracted/snapshot-dependencies/ ./
COPY --from=build /out/extracted/application/ ./

ENV SERVER_PORT=8080
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["java", "-jar", "/app/app.jar"]
```

The environment variable and `RUN` commands are configured as follows:

- `chmod +x mvnw` makes the copied Maven Wrapper script executable in the build image. `./mvnw` then invokes the Maven distribution pinned in the wrapper configuration.
- `-B` runs Maven in batch mode without interactive prompts, which is suitable for automated image builds. `-ntp` disables transfer-progress output, keeping build logs less noisy.
- `dependency:go-offline` resolves and downloads project dependencies and build plugins before source is copied. This places the relatively stable downloads in a layer that can be reused when application code changes.
- `./mvnw -B -ntp verify` runs the Maven `verify` lifecycle, including preceding build phases and checks bound by the project. The shared `-B` and `-ntp` options retain noninteractive, quieter build logs.
- `&&` runs the following file operations only if Maven succeeds. `mkdir -p /out` creates the artifact directory and succeeds if it already exists. `cp target/application.jar /out/app.jar` gives the built executable JAR a stable filename. Spring Boot's tools jarmode then extracts its dependency and application layers into `/out/extracted` for separate runtime-stage copies.
- `SERVER_PORT=8080` sets Spring Boot's default HTTP listening port through environment-variable configuration. It matches `EXPOSE 8080` and can be overridden at runtime.

Build from the Maven project root. The wrapper distribution and plugin versions are controlled by the checked-in wrapper configuration and POM. Add a `.dockerignore` for `.git`, IDE files, local settings, and build output.

Example `.dockerignore`:

```dockerignore
.git
.env*
.idea/
*.iml
target/
```

If the builder supports cache mounts, cache Maven's local repository at `/root/.m2/repository`.

## Choices and Tradeoffs

- Spring Boot's layered JAR extraction keeps dependencies, the Boot loader, and application files in separate image layers. Routine application changes can then reuse dependency layers. This requires a Spring Boot executable JAR containing the tools jarmode (Spring Boot 3.3+); a plain fat-JAR copy is simpler when layer reuse is not important.
- The build stage runs Maven and tests; the runtime stage contains only a JRE and the extracted executable application. A JRE is sufficient for a conventional Spring Boot JVM deployment. A native image is a separate build strategy with different build tooling, compatibility, and startup tradeoffs.
- The wrapper pins the Maven distribution for the project. Apache Maven 3.10.0 is the current stable release checked for this guide. Copying the POM and wrapper configuration before source allows Maven dependency downloads to be cached, although `dependency:go-offline` may not resolve every plugin dependency and `verify` can still need network access. Run `verify` rather than silently skipping tests in the representative production build. Tests requiring Docker/Testcontainers or external services belong in CI outside the image build, or should be separated into a profile that can be excluded deliberately.
- A numeric non-root user avoids depending on a vendor-specific account. The JAR is readable by that user; use a separately owned writable directory if the application needs file output. Most API configuration can be supplied through Spring Boot's environment-variable binding, such as `SERVER_PORT`; credentials remain runtime-provided secrets.
- `ENTRYPOINT` uses exec form so the JVM receives termination signals. Port 8080 is the documented default and can be changed through Spring configuration.
- The JVM's default maximum heap is based on a fraction of the container memory limit (commonly 25%). Set `JAVA_TOOL_OPTIONS=-XX:MaxRAMPercentage=...` at runtime when the default heap allocation does not fit the application's total memory budget.
- Configure a meaningful health probe only when the application exposes one; the runtime image may not include `curl`, so use a suitable in-image probe or a platform-native check.

## Validation Recipe

The runnable sample for this guide is in [`examples/java-spring-boot/`](../examples/java-spring-boot/). From that directory, build and run the image, verify its runtime identity and layered application contents, exercise both routes, then stop it cleanly:

```sh
docker build -t spring-api .
docker image inspect --format '{{.Config.User}}' spring-api
docker create --name spring-api-inspect spring-api
docker export spring-api-inspect | tar -tf - | grep -E '^(app.jar|BOOT-INF/lib/|org/springframework/boot/loader/)' | head
docker rm spring-api-inspect
docker run -d --name spring-api -p 8080:8080 spring-api
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /tmp/spring-root.html --write-out '%{http_code}\n' http://127.0.0.1:8080/ | grep -qx 200
grep -q 'Hello World' /tmp/spring-root.html
curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-all-errors --retry-delay 1 --output /dev/null --write-out '%{http_code}\n' http://127.0.0.1:8080/healthz | grep -qx 200
docker stop --time 10 spring-api
docker logs spring-api | grep -q 'Graceful shutdown complete'
docker rm spring-api
```

The image should report user `10001:10001`; the extracted filesystem should contain the executable application and dependencies, but not Maven, the JDK, or source. `/` should return HTML containing `Hello World`, `/healthz` should return HTTP 200, and the JVM should stop cleanly on the container's termination signal.

## References

- [Spring Boot 4.1 system requirements](https://docs.spring.io/spring-boot/system-requirements.html)
- [OpenJDK 25 project page](https://openjdk.org/projects/jdk/25/)
- [Apache Maven downloads and release](https://maven.apache.org/download.cgi)
- [Eclipse Temurin images](https://hub.docker.com/_/eclipse-temurin)