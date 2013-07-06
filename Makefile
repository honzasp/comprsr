RUSTC = rustc
RUSTC_FLAGS         = -L . -A non-uppercase-statics
RUSTC_TEST_FLAGS    = $(RUSTC_FLAGS) 
RUSTC_COMPILE_FLAGS = $(RUSTC_FLAGS)

COMPRSR_SRCS = $(shell find src -name '*.rs' -type f)
ALL_CRATES   = $(shell find src -name '*.rc')
ALL_DUMMIES  = $(shell find src -name '*.rc' | sed 's/src\/comprsr_\([a-zA-Z0-9]*\)\.rc/libcomprsr_\1.dummy/')
ALL_TESTS    = $(shell find src -name '*.rc' | sed 's/src\/comprsr_\([a-zA-Z0-9]*\)\.rc/test_\1/')
ALL_BENCHS   = $(shell find src -name '*.rc' | sed 's/src\/comprsr_\([a-zA-Z0-9]*\)\.rc/bench_\1/')

.PHONY: all all_tests unit_tests benchmarks func_tests clean

all: $(ALL_DUMMIES)

all_tests: unit_tests benchmarks func_tests 
unit_tests: $(ALL_TESTS)
benchmarks: $(ALL_BENCHS)

func_tests: libcomprsr_zlib.dummy
	cd test; $(MAKE) all

libcomprsr_%.dummy: src/comprsr_%.rc src/%/*.rs
	$(RUSTC) $(RUSTC_COMPILE_FLAGS) $< --out-dir .
	touch $@

test_%: testcomprsr_%~
	./$<

bench_%: testcomprsr_%~
	./$< --bench

testcomprsr_%~: src/comprsr_%.rc src/%/*.rs
	$(RUSTC) $(RUSTC_TEST_FLAGS) --test $< -o $@

clean:
	rm -f testcomprsr_*~ libcomprsr_*.dummy libcomprsr_*.so

libcomprsr_zlib.dummy testcomprsr_zlib~: libcomprsr_inflate.dummy libcomprsr_checksums.dummy libcomprsr_bits.dummy

libcomprsr_gzip.dummy testcomprsr_gzip~: libcomprsr_inflate.dummy libcomprsr_checksums.dummy libcomprsr_bits.dummy

libcomprsr_checksums.dummy testcomprsr_checksums~: libcomprsr_bits.dummy

libcomprsr_inflate.dummy testcomprsr_inflate~: libcomprsr_bits.dummy
