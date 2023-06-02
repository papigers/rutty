
ifdef release
	export NODE_ENV = production
	CARGO_ARGS = --release
	TARGET = release
else
	export NODE_ENV = development
	PARCEL_ARGS = --no-optimize
	TARGET = debug
endif

web/dist/$(TARGET): web/package.json web/index.html $(wildcard web/js/**/*) $(wildcard web/css/**/*)
	rm -rf web/dist
	cd web;\
	yarn --production=false;\
	yarn build $(PARCEL_ARGS) --dist-dir dist/$(TARGET)

backend/static/index.html: web/dist/$(TARGET)
	rm -rf backend/static/**/*
	cp -r web/dist/$(TARGET) backend/static

target/$(TARGET): backend/static/index.html $(wildcard backend/src/**/*) backend/Cargo.toml
	rm -rf target/$(TARGET)
	cargo build $(CARGO_ARGS)

build: target/$(TARGET)
.PHONY: build

clean:
	rm -rf backend/static target/$(TARGET) web/dist
clean-full: clean
	rm -rf web/.parcel-cache web/node_modules
.PHONY: clean clean-full