#This only compiles a partial cross compiler at the moment
NEWLIBv="newlib-1.16.0"
SCRIPT_DIR="$PWD"

export PREFIX=/usr/local
export TARGET=i386-elf-doors

cd "$SCRIPT_DIR"
mkdir build-newlib

rm -rf "$SCRIPT_DIR"/build-newlib/*

cd "$SCRIPT_DIR"/"$NEWLIBv"/newlib/libc/sys
autoconf

cd "$SCRIPT_DIR"/"$NEWLIBv"/newlib/libc/sys/doors
autoreconf

cd "$SCRIPT_DIR"

cd "$SCRIPT_DIR"/build-newlib
"$SCRIPT_DIR"/"$NEWLIBv"/configure --prefix=$PREFIX --target=$TARGET
make all
make install

