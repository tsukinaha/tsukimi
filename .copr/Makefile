SPEC := $(if $(spec),$(spec),tsukimi.spec)

.PHONY: srpm
srpm:
	dnf -y install cargo git-core python3 rpm-build rust zstd
	name="$$(rpmspec -q --srpm --qf '%{NAME}\n' "$(SPEC)" | head -n1)"; \
	if [ "$$name" = "tsukimi-git" ]; then \
		version="$$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')"; \
		git_date="$$(git log -1 --date=format:%Y%m%d --format=%cd HEAD)"; \
		git_short="$$(git rev-parse --short=10 HEAD)"; \
	else \
		tag="$$(git describe --tags --exact-match HEAD 2>/dev/null || true)"; \
		if [ -n "$$tag" ]; then \
			version="$${tag#v}"; \
		else \
			version="$$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')"; \
		fi; \
	fi; \
	workdir="$$(mktemp -d)"; \
	mkdir -p "$$workdir/.cargo"; \
	git archive --format=tar.gz --prefix="$$name-$$version/" -o "$(outdir)/$$name-$$version.tar.gz" HEAD; \
	cargo vendor --locked "$$workdir/vendor" > "$$workdir/.cargo/config.toml"; \
	sed -i 's|^directory = \".*\"|directory = \"vendor\"|' "$$workdir/.cargo/config.toml"; \
	tar --zstd -cf "$(outdir)/$$name-$$version-vendor.tar.zst" -C "$$workdir" .cargo vendor; \
	if [ "$$name" = "tsukimi-git" ]; then \
		rpmbuild -bs "$(SPEC)" \
			--define "version_from_git $$version" \
			--define "git_snapshot $${git_date}git$${git_short}" \
			--define "_sourcedir $(outdir)" \
			--define "_srcrpmdir $(outdir)"; \
	else \
		rpmbuild -bs "$(SPEC)" \
			--define "version_from_tag $$version" \
			--define "_sourcedir $(outdir)" \
			--define "_srcrpmdir $(outdir)"; \
	fi; \
	rm -rf "$$workdir"
