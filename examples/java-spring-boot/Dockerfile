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