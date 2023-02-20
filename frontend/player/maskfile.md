# Frontend Player Tasks

## build

> Build the project

```bash
set -e
if [[ "$verbose" == "true" ]]; then
    set -x
fi

cd $MASKFILE_DIR

wasm-pack build --target web --out-name player --out-dir ./pkg --release
```

### dev

> Run the project in development mode

**OPTIONS**

- watch
  - flags: --watch
  - desc: Watch for changes and rebuild

```bash
cd $MASKFILE_DIR

wasm-pack build --target web --out-name player --out-dir ./pkg --dev

if [ "$watch" == "true" ]; then
    while true; do
        cargo-watch -q --postpone -s "wasm-pack build --target web --out-name player --out-dir ./pkg --dev"
    done
fi
```

## clean

> Clean the project

```bash
cd $MASKFILE_DIR

rm -rf pkg
```
