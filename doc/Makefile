all: html man
.PHONY: all html man

html:
	DESTDIR="$(shell pwd)/../target/doc/cli" make -C ../crates/cli/doc html

man:
	DESTDIR="$(shell pwd)/../target/doc/cli" make -C ../crates/cli/doc man
