# syntax=docker/dockerfile:1.7

FROM node:24-trixie-slim

ENV NODE_ENV=production \
    PORT=3000

WORKDIR /app

COPY package.json package-lock.json ./
RUN --mount=type=secret,id=npm-ca,target=/run/secrets/npm-ca,required=false \
    if [ -s /run/secrets/npm-ca ]; then \
        npm ci --omit=dev --cafile=/run/secrets/npm-ca; \
    else \
        npm ci --omit=dev; \
    fi && npm cache clean --force

COPY src/ ./src/

EXPOSE 3000
USER node
CMD ["node", "src/index.js"]