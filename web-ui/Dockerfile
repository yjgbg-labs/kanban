FROM node:22-slim AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --legacy-peer-deps
COPY . .
RUN npm run build && tar cf /build.tar build

FROM node:22-slim
WORKDIR /app
COPY package*.json ./
RUN npm ci --legacy-peer-deps --omit=dev
RUN --mount=type=bind,from=builder,source=/build.tar,target=/tmp/build.tar tar xf /tmp/build.tar
EXPOSE 3000
CMD ["npm", "start"]
