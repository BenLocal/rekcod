build: web
  cargo clean
  just cross x86_64-unknown-linux-musl
  just cross aarch64-unknown-linux-musl

web:
  cd rekcod-dashboard/app && \
  npm install && \
  VITE_PUBLIC_PATH=rekcod npm run build

cross target: 
  cross build --release --target {{ target }}

tidy:
  cargo machete --fix

pkg target:
  rm -rf release/{{ target }} && mkdir -p release/{{ target }}
  cp ./target/{{ target }}/release/rekcod release/{{ target }}/rekcod
  cp ./target/{{ target }}/release/rekcodd release/{{ target }}/rekcodd
  cp ./scripts/** release/{{ target }}/

release: build
  just pkg x86_64-unknown-linux-musl
  just pkg aarch64-unknown-linux-musl

ins:
  cd ./release/x86_64-unknown-linux-musl && chmod +x install.sh && ./install.sh server
