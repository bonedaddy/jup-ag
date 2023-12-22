.PHONY: fmt
fmt:
	find -type f -name "*.rs" -not -path "*target*" -not -path "*vendor*" -exec rustfmt --edition 2021 {} \;
