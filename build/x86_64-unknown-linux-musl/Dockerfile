FROM ghcr.io/cross-rs/x86_64-unknown-linux-musl:main

ENV SSL_VER="1.1.1q" \
    PREFIX=/musl-x86_64 \
    OPRNSSL_HOST="linux-x86_64"

# build openssl
RUN curl -sSL https://www.openssl.org/source/openssl-"$SSL_VER".tar.gz | tar xz && \
    cd openssl-"$SSL_VER" && \
    env CC="$CC_x86_64_unknown_linux_musl" ./Configure no-zlib no-shared -fPIC --prefix="$PREFIX" --openssldir="$PREFIX"/ssl "$OPRNSSL_HOST" && \
    env C_INCLUDE_PATH="$PREFIX"/include make depend 2> /dev/null && \
    make -j$(nproc) && make install && \
    cd .. && rm -rf openssl-"$SSL_VER"

ENV OPENSSL_STATIC=true \
    OPENSSL_DIR=$PREFIX \
    OPENSSL_INCLUDE_DIR=$PREFIX/include \
    OPENSSL_LIB_DIR=$PREFIX/lib