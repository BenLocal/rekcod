FROM ghcr.io/cross-rs/aarch64-unknown-linux-musl:main

ENV SSL_VER="1.1.1q" \
    PREFIX=/musl-aarch64 \
    OPRNSSL_HOST="linux-aarch64"

# build openssl
ARG OPENSSL_HOST="https://www.openssl.org"
RUN curl -sSL ${OPENSSL_HOST}/source/openssl-"$SSL_VER".tar.gz | tar xz && \
    cd openssl-"$SSL_VER" && \
    env CC="$CC_aarch64_unknown_linux_musl" ./Configure no-zlib no-shared -fPIC --prefix="$PREFIX" --openssldir="$PREFIX"/ssl "$OPRNSSL_HOST" && \
    env C_INCLUDE_PATH="$PREFIX"/include make depend 2> /dev/null && \
    make -j$(nproc) && make install && \
    cd .. && rm -rf openssl-"$SSL_VER"

ENV OPENSSL_STATIC=true \
    OPENSSL_DIR=$PREFIX \
    OPENSSL_INCLUDE_DIR=$PREFIX/include \
    OPENSSL_LIB_DIR=$PREFIX/lib
