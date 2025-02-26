#!/usr/bin/env fish

set OUTPUT_DIR cim_sweep

set N 16 32 64 128 256
set M 16 32 64 128 256

set BL '1, 2, 0, -1'
set WL '4, 2.5, 0, 1'
set WELL '0, 4'
set CELL 1FeFET_100

# Optional parameters
set ENOB 6
set FS 100e6

# ADCs: Set to 'all' to match BLs, otherwise specify num
set ADCS all

function writeout -d "Write one configuration file"
	set n $argv[1]
	set m $argv[2]
	set filename $OUTPUT_DIR/$n-$m.txt

	if test -f $filename
		read -P "Output file $filename already exists, overwrite it? (Y/n) " allow

		if test (string lower $allow) != 'y'
			and test -n "$allow"
			echo "Aborting..."
			exit 2
		end

		/bin/rm $filename
	end

	echo "n: $n" >> $filename
	echo "m: $m" >> $filename

	echo "bl: $BL" >> $filename
	echo "wl: $WL" >> $filename
	echo "well: $WELL" >> $filename
	echo "cell: $CELL" >> $filename

	if test -n $ENOB 
		echo "enob: $ENOB" >> $filename
	end

	if test -n $FS
		echo "fs: $FS" >> $filename
	end

	if test $ADCS = 'all'
		echo "adcs: $BL" >> $filename
	end
	if test -n $ADCS
		echo "adcs: $ADCS" >> $filename
	end

end

mkdir -p $OUTPUT_DIR

# Generate all possible combinations of n and m
for n in $N
	for m in $M
		writeout $n $m
	end
end
