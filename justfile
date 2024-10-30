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