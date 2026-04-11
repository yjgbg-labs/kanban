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
COPY --from=builder /build.tar /tmp/
RUN tar xf /tmp/build.tar && rm /tmp/build.tar
EXPOSE 3000
CMD ["npm", "start"]
