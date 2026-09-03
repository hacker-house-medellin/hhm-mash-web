# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.97
FROM --platform=$TARGETPLATFORM rust:${RUST_VERSION}-alpine AS build

ARG LAMBDA_BIN
ARG TARGETARCH
ENV CARGO_INCREMENTAL=0
ENV RUSTFLAGS="-C debuginfo=0 --remap-path-prefix=/work=."

WORKDIR /work
RUN apk add --no-cache build-base file git

COPY Cargo.toml ./
COPY src ./src

RUN set -eu; \
    case "${TARGETARCH}" in \
      amd64|arm64) ;; \
      *) echo "unsupported TARGETARCH=${TARGETARCH}; expected amd64 or arm64" >&2; exit 64 ;; \
    esac; \
    test -n "${LAMBDA_BIN}"; \
    cargo build --release --bin "${LAMBDA_BIN}"; \
    file "target/release/${LAMBDA_BIN}" | tee /tmp/lambda-file.txt; \
    grep -Eq 'statically linked|static-pie linked' /tmp/lambda-file.txt; \
    install -D -m 0555 "target/release/${LAMBDA_BIN}" /out/bootstrap; \
    test "$(ls -A /out)" = "bootstrap"

FROM scratch AS artifact
COPY --from=build /out/bootstrap /bootstrap

FROM public.ecr.aws/lambda/provided:al2023 AS runtime
COPY --from=build --chmod=0555 /out/bootstrap /var/runtime/bootstrap
ENTRYPOINT ["/var/runtime/bootstrap"]
