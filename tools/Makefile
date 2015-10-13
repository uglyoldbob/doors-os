PATH:=$(PATH):$(PREFIX)/bin

all: gcc2_t

check_compiler_source: newlib_libc_sys newlib_libc_sys_doors gcc_libstdcplusplus
	echo "make all done"

newlib_libc_sys: $(shell find newlib/newlib/libc/sys -type f)
	cd newlib/newlib/libc/sys; autoconf
	touch newlib_libc_sys

newlib_libc_sys_doors: $(shell find newlib/newlib/libc/sys/doors -type f)
	cd newlib/newlib/libc/sys/doors; autoconf2.64
	touch newlib_libc_sys_doors

gcc_libstdcplusplus: $(shell find gcc/libstdc++-v3 -type f)
	cd gcc/libstdc++-v3; autoconf2.64
	touch gcc_libstdcplusplus

binutils_t: $(shell find binutils -type f)
	mkdir -p build_binutils
	cd build_binutils; ../binutils/configure --target=$(TARGET) --prefix=$(PREFIX)
	cd build_binutils; make clean all
	cd build_binutils; sudo make install
	touch binutils_t

gcc_t: binutils_t $(shell find gcc -type f)
	mkdir -p build_gcc
	cd build_gcc; ../gcc/configure --target=$(TARGET) --prefix=$(PREFIX) --disable-nls --enable-languages=c,c++ --with-newlib
	cd build_gcc; make clean all-gcc
	cd build_gcc; sudo make install-gcc
	touch gcc_t

newlib_t: gcc_t $(shell find newlib -type f)
	mkdir -p build_newlib
	cd build_newlib; ../newlib/configure --prefix=$(PREFIX) --target=$(TARGET)
	cd build_newlib; make clean all
	cd build_newlib; sudo make install
	touch newlib_t

gcc2_t: newlib_t gcc_t
	cd build_gcc; make
	cd build_gcc; sudo make install
	touch gcc2_t
