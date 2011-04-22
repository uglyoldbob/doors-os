#This only compiles a partial cross compiler at the moment

export PREFIX=/home/thomas/doors-os/cross
export TARGET=i386-elf-doors
export LOCATE=/home/thomas/tools

mkdir build-binutils build-gcc build-newlib

cd $LOCATE/newlib-1.15.0/newlib/libc/sys
autoconf

cd $LOCATE/newlib-1.15.0/newlib/libc/sys/doors
autoreconf

cd $LOCATE

export PATH=$PATH:$PREFIX/bin

#be sure to copy sources for the binutils and gcc into /usr/src first
#be sure to change the numbers as necessary

echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!
echo BINUTILS!

#binutils
cd build-binutils
#../binutils-2.18/configure --target=$TARGET --prefix=$PREFIX  --disable-nls
../binutils-2.18/configure --target=$TARGET --prefix=$PREFIX
make all
make install

echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!
echo GCC!

#gcc
cd ../build-gcc
export PATH=$PATH:$PREFIX/bin
#../gcc-4.2.2/configure --target=$TARGET --prefix=$PREFIX --without-headers --disable-nls --enable-languages=c
../gcc-4.2.2/configure --target=$TARGET --prefix=$PREFIX --disable-nls --enable-languages=c
make all-gcc 
make install-gcc

echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!
echo NEWLIB!


cd ../build-newlib
../newlib-1.15.0/configure --prefix=$PREFIX --target=$TARGET
make all
make install


#cd ../build-gcc
#make
#make install

#cd ../build-gcc
#../gcc-4.2.2/configure --target=$TARGET --prefix=$PREFIX --with-newlib --enable-languages=c
#make all
#make install
