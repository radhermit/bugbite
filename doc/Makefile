all: man
.PHONY: all man

man:
	DESTDIR="$(shell pwd)/../target/doc/cli" make -C ../crates/cli/doc
