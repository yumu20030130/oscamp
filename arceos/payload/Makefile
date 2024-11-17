SUB_DIRS=origin hello_c

all: $(SUB_DIRS)

$(SUB_DIRS): FORCE
	make -C $@

FORCE:

.PHONY: all SUB_DIRS FORCE
