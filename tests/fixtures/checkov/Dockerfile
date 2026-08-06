FROM nginx:1.29.0-alpine

HEALTHCHECK \
    --interval=30s \
    --timeout=5s \
    --start-period=10s \
    --retries=3 \
    CMD wget --quiet --spider "http://127.0.0.1/" || exit 1

USER nginx

EXPOSE 8080

