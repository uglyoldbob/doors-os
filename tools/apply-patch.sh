SCRIPT_DIR="$PWD"

#todo parse arguments so this script doesn't have to be modified
NEWLIBv="newlib-1.16.0"
NEWLIBf="newlib-1.16.0.tar.gz"
GCCv="gcc-4.3.2"
GCCf="gcc-4.3.2.tar.bz2"
BINUTILSv="binutils-2.18"
BINUTILSf="binutils-2.18.tar.gz"

#genpatch(directory, archive, patchname)
#need to find the best way of removing files created by compiling the package (make clean/whatever)
applypatch()
{
	tar -xf $2;
	cp $1 $1.mod;
	patch -p0 <$3;
	rm $1;
	mv $1.mod $1;
}

#creates all patches required
#diff -Naur old-dir new-dir > patchname
#patch -p0 <patch-name

#applypatch "$SCRIPT_DIR"/"$NEWLIBv" 	"$SCRIPT_DIR"/../source/"$NEWLIBf"	 	patch/"$NEWLIBv"-patch
applypatch "$SCRIPT_DIR"/"$GCCv"		"$SCRIPT_DIR"/../source/"$GCCf"			patch/"$GCCv"-patch
#applypatch "$SCRIPT_DIR"/"$BINUTILSv"	"$SCRIPT_DIR"/../source/"$BINUTILSf"	patch/"$BINUTILSv"-patch

