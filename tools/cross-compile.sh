#This compiles a cross compiler at the moment

NEWLIBv="newlib-1.16.0"
GCCv="gcc-4.3.2"
BINUTILSv="binutils-2.18"
SCRIPT_DIR="$PWD"

export PREFIX=/usr/local
export TARGET=i386-pc-doors

sudo rm -rf builf-binutils
sudo rm -rf build-gcc
sudo rm -rf build-newlib
mkdir build-binutils build-gcc build-newlib

echo cd "$SCRIPT_DIR"/"$NEWLIBv"/newlib/libc/sys
cd "$SCRIPT_DIR"/"$NEWLIBv"/newlib/libc/sys
autoconf

echo cd "$SCRIPT_DIR"/"$NEWLIBv"/newlib/libc/sys/doors
cd "$SCRIPT_DIR"/"$NEWLIBv"/newlib/libc/sys/doors
autoreconf


echo cd "$SCRIPT_DIR"/"$GCCv"/libstdc++-v3
cd "$SCRIPT_DIR"/"$GCCv"/libstdc++-v3
autoconf


cd "$SCRIPT_DIR"

export PATH=$PATH:$PREFIX/bin

#be sure to copy sources for the binutils and gcc into /usr/src first
#be sure to change the numbers as necessary

echo BINUTILS!

#binutils
cd "$SCRIPT_DIR"/build-binutils
#"$SCRIPT_DIR"/"$BINUTILSv"/configure --target=$TARGET --prefix=$PREFIX  --disable-nls
"$SCRIPT_DIR"/"$BINUTILSv"/configure --target=$TARGET --prefix=$PREFIX
make clean all
sudo make install

echo GCC!

#gcc
cd "$SCRIPT_DIR"/build-gcc
export PATH=$PATH:$PREFIX/bin
#"$SCRIPT_DIR"/"$GCCv"/configure --target=$TARGET --prefix=$PREFIX --without-headers --disable-nls --enable-languages=c
"$SCRIPT_DIR"/"$GCCv"/configure --target=$TARGET --prefix=$PREFIX --disable-nls --enable-languages=c,c++
make clean all-gcc 
sudo make install-gcc

echo NEWLIB!

cd "$SCRIPT_DIR"/build-newlib
"$SCRIPT_DIR"/"$NEWLIBv"/configure --prefix=$PREFIX --target=$TARGET
make clean all
sudo make install

echo GCC!

cd "$SCRIPT_DIR"/build-gcc
make
sudo make install

