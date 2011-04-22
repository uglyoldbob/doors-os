SCRIPT_DIR="$PWD"

#this script should only be needed when preparing a patch for a different version of the tools

#genpatch(directory, archive, patchname)
#this script is used to ease the task of producing patch files for version of gcc, binutils, and newlib
#unpack the source, apply the patch by hand, then run this script
#archive goes in doorsos/source

#to modify the existing patch
#run apply-patch to create a "blank" source folder
#apply changes by hand
#run create-patch

genpatch()
{
	mv $1 $1.mod;
	tar -xf $2;
	diff -Naur $1 $1.mod > $3;
	rm -rf $1;
	mv $1.mod $1;
}

#creates all patches required
#diff -Naur old-dir new-dir > patchname
#applies patches
#patch -p0 <patch-name

genpatch "$SCRIPT_DIR"/newlib-1.15.0 	"$SCRIPT_DIR"/../source/newlib-1.15.0.tar.gz 	newlib-1.15.0-patch
#genpatch "$SCRIPT_DIR"/gcc-4.3.2		"$SCRIPT_DIR"/../source/gcc-4.3.2.tar.bz2		gcc-4.3.2-patch
