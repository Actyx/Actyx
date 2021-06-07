FROM node:10-alpine as build
WORKDIR /usr/src/app

RUN apk add --update gcc g++ libc-dev python3 make
COPY src/wago-connector/package-docker.json ./package.json

RUN npm install --production
COPY build/wago-connector/. .


FROM node:10-alpine
COPY --from=build /usr/src/app /
CMD ["node", "wago-connector/index.js"]
