# Build dashboard
FROM node:lts-alpine as build

RUN apk update && \
    apk upgrade && \
    apk add --update git

WORKDIR /app

ADD . sfy-dashboard
WORKDIR /app/sfy-dashboard

# ENV SFY_SERVER=https://wavebug.k8s.met.no
ENV SFY_SERVER=WAVEBUG_SERVER_URL

RUN npm install
RUN npm run build

FROM alpine:latest

COPY files/ /

RUN apk update && \
    apk upgrade && \
    apk add --update nginx bash && \
    chown -R root:nginx /etc/nginx && \
    chmod -R a+rX /etc/nginx && \
    chmod -R a+rwX /var/lib/nginx && \
    rm -rf /etc/nginx/conf.d/default.conf /var/cache/apk/*

# COPY --from=build /app/sfy/sfy-dashboard/build /usr/share/nginx/html
RUN mkdir /sfy-dashboard
COPY --from=build /app/sfy-dashboard/build /sfy-dashboard

EXPOSE 8080
USER nginx:nginx
CMD ["/usr/sbin/nginx", "-c", "/etc/nginx/override.conf"]
