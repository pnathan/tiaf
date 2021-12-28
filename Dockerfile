# pin that version
FROM alpine:3.15.0 as alpine

RUN apk add -U --no-cache ca-certificates

FROM scratch
WORKDIR /
COPY --from=alpine /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY tiaf /tiaf
CMD ["/tiaf"]
