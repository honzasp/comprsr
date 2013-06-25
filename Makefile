RUSTC = rustc
TOUCH = touch

RUST_FLAGS = 
COMPRSR_SRCS = $(shell find src -name '*.rs' -type f)

.PHONY: all test clean

all: libcomprsr.dummy

libcomprsr.dummy: src/comprsr.rc $(COMPRSR_SRCS)
	$(RUSTC) $(RUST_FLAGS) $< --out-dir .
	$(TOUCH) $@

test: testcomprsr~
	./$<

testcomprsr~: src/comprsr.rc $(COMPRSR_SRCS)
	$(RUSTC) $(RUST_FLAGS) --test $< -o $@

clean:
	rm -f testcomprsr~ libcomprsr.dummy libcomprsr-*.so
