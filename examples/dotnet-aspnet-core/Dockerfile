FROM mcr.microsoft.com/dotnet/sdk:10.0-noble AS build

WORKDIR /src
COPY Api.csproj packages.lock.json ./
RUN dotnet restore Api.csproj --locked-mode

COPY . ./
RUN dotnet publish Api.csproj \
    --no-restore \
    --configuration Release \
    --output /out \
    /p:UseAppHost=false

FROM mcr.microsoft.com/dotnet/aspnet:10.0-noble

WORKDIR /app
COPY --from=build /out/ ./

ENV ASPNETCORE_HTTP_PORTS=8080
USER $APP_UID
EXPOSE 8080
ENTRYPOINT ["dotnet", "Api.dll"]