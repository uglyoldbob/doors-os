#This only compiles a partial cross compiler at the moment

export PREFIX=/home/thomas/doors-os/cross
export TARGET=i386-elf-doors
export LOCATE=/home/thomas/tools

cd $LOCATE
mkdir build-newlib

rm -rf $LOCATE/build-newlib/*

cd $LOCATE/newlib-1.15.0/newlib/libc/sys
autoconf

cd $LOCATE/newlib-1.15.0/newlib/libc/sys/doors
autoreconf

cd $LOCATE

cd $LOCATE/build-newlib
../newlib-1.15.0/configure --prefix=$PREFIX --target=$TARGET
make all
make install

