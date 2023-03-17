mkdir build-bochs
rm -rf /home/thomas/tools/build-bochs/*

cd build-bochs

../bochs-2.3.6/configure --enable-cpu-level=3 \
						--enable-disasm \
						--enable-debugger \
#						--enable-iodebug \
						--enable-x86-debugger  \
#						--enable-gdb-stub
make
sudo make install
