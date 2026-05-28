Name:           resolved-nftset
Version:        %{?pkg_version}
Release:        1%{?dist}
Summary:        Service that adds resolved IPs to nftable sets
License:        MIT or APACHE-2.0
URL:            https://github.com/localvoid/resolved-nftset

BuildRequires:  systemd-rpm-macros
%{?systemd_requires}

%description
%{name} is service that adds resolved IPs to nftable sets:

1. Watches every DNS query resolved via systemd-resolved resolver.
2. Matches the queried hostnames against a user-defined ruleset.
3. On a match, adds the resolved IPs to a user-defined nftables set.

%preun
systemctl disable --now "%{name}.service" --no-warn

%build

%install
install -D -m 755 -d %{buildroot}%{_sysconfdir}/%{name}
install -D -m 755 %{_sourcedir}/%{name} %{buildroot}%{_bindir}/%{name}
install -D -m 644 %{_sourcedir}/%{name}.service %{buildroot}%{_unitdir}/%{name}.service

%files
%dir %attr(755, root, root) %{_sysconfdir}/%{name}
%attr(755, root, root) %{_bindir}/%{name}
%attr(644, root, root) %{_unitdir}/%{name}.service
