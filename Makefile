RUSTC = rustc
TOUCH = touch
RUSTC_FLAGS         = -L .
RUSTC_TEST_FLAGS    = $(RUSTC_FLAGS) 
RUSTC_COMPILE_FLAGS = $(RUSTC_FLAGS)

COMPRSR_SRCS = $(shell find src -name '*.rs' -type f)
ALL_CRATES   = $(shell find src -name '*.rc')
ALL_DUMMIES  = $(shell find src -name '*.rc' | sed 's/src\/comprsr_\([a-zA-Z0-9]*\)\.rc/libcomprsr_\1.dummy/')
ALL_TESTS    = $(shell find src -name '*.rc' | sed 's/src\/comprsr_\([a-zA-Z0-9]*\)\.rc/test_\1/')

.PHONY: all all_tests clean

all: $(ALL_DUMMIES)
all_tests: $(ALL_TESTS)

libcomprsr_%.dummy: src/comprsr_%.rc src/%/*.rs
	$(RUSTC) $(RUSTC_COMPILE_FLAGS) $< --out-dir .
	$(TOUCH) $@

test_%: testcomprsr_%~
	./$<

testcomprsr_%~: src/comprsr_%.rc src/%/*.rs
	$(RUSTC) $(RUSTC_TEST_FLAGS) --test $< -o $@

clean:
	rm -f testcomprsr_*~ libcomprsr_*.dummy libcomprsr_*.so

libcomprsr_zlib.dummy testcomprsr_zlib~: libcomprsr_inflate.dummy
