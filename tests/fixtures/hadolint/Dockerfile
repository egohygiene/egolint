# syntax=docker/dockerfile:1

FROM docker.io/library/alpine:3.20.3

SHELL ["/bin/ash", "-eo", "pipefail", "-c"]

RUN addgroup --system app \
    && adduser --system --ingroup app app

WORKDIR /home/app

USER app

CMD ["sh", "-c", "printf '%s\n' 'Hadolint fixture passed.'"]

