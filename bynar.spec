Name: bynar
Version: 0.1
Release: 2%{?dist}
Summary: Bynar Automated Hardware Repair

License: Apache2
URL: https://github.com/Comcast/Bynar

%define debug_package %{nil}

%{?systemd_requires}
BuildRequires: systemd

Requires: librados2

%description
Bynar is an open source system for automating server maintenance across the datacenter. Bynar builds upon many years of experience automating the drudgery of server repair.

%prep

%install
rm -rf $RPM_BUILD_ROOT
mkdir $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/usr/sbin $RPM_BUILD_ROOT/lib/systemd/system $RPM_BUILD_ROOT/etc/bynar

cp $RPM_BUILD_DIR/target/release/bynar $RPM_BUILD_ROOT/usr/sbin/bynar
cp $RPM_BUILD_DIR/target/release/bynar-client $RPM_BUILD_ROOT/usr/bin/bynar-client
cp $RPM_BUILD_DIR/target/release/disk-manager $RPM_BUILD_ROOT/usr/sbin/disk-manager

cp $RPM_BUILD_DIR/config/bynar.json $RPM_BUILD_ROOT/etc/bynar/bynar.json
cp $RPM_BUILD_DIR/config/disk-manager.json $RPM_BUILD_ROOT/etc/bynar/disk-manager.json
cp $RPM_BUILD_DIR/config/ceph.json $RPM_BUILD_ROOT/etc/bynar/ceph.json

cp $RPM_BUILD_DIR/systemd/disk-manager.service $RPM_BUILD_ROOT/lib/systemd/system

%files
/usr/sbin/bynar
/usr/bin/bynar-client
/usr/sbin/disk-manager
/lib/systemd/system/disk-manager.service
%dir /etc/bynar
%config(noreplace) /etc/bynar/bynar.json
%config(noreplace) /etc/bynar/ceph.json
%config(noreplace) /etc/bynar/disk-manager.json

%doc

%changelog

%post
%systemd_post disk-manager.service

%preun
%systemd_preun disk-manager.service

%postun
%systemd_postun_with_restart disk-manager.service
