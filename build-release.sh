#!/bin/bash
if [ -z "$1" ]
  then
    echo "No argument supplied, requires build version"
    exit 1
fi

set -euo pipefail

distro=$1
path=`pwd`
echo "About to launch $distro container"
container="bynar-build-$RANDOM"

#function finish {
#    echo "Cleaning up: ($?)!"
#    docker kill ${container}
#	sleep 5
#    docker rm ${container}
#    echo "finished cleaning up"
#}
#trap finish EXIT

echo "Named container: ${container}"
docker run --name ${container} -d -i -t -v $path:/build -w /build $distro
echo "Launched ${container}"

echo "Installing deps"
if [[ "$distro" == centos* ]]
    then
	docker exec ${container} yum update -y
	docker exec ${container} yum install --nogpgcheck -y epel-release
	echo "installing"
    packages="libatasmart-devel openssl-devel librados2-devel centos-release-scl"
	docker exec ${container} yum install -y $packages
	docker exec ${container} yum install -y llvm-toolset-7
fi

if [[ "$distro" == ubuntu* ]]
    then
	docker exec ${container} apt update
	echo "installing "
  packages="gcc curl libblkid-dev liblvm2-dev liblvm2app2.2 libdevmapper-dev libzmq5 libatasmart-dev libssl-dev librados-dev libudev-dev libzmq3-dev make pkg-config"
	docker exec ${container} apt-get install -y $packages
fi

echo "About to install rust"
docker exec ${container} curl https://sh.rustup.rs -o /root/rustup.sh
echo "chmod"
docker exec ${container} chmod +x /root/rustup.sh
echo "installing rust"
docker exec ${container} /root/rustup.sh -y

echo "Building"
if [[ "$distro" == centos* ]]
	then
	docker exec ${container} scl enable llvm-toolset-7 '/root/.cargo/bin/cargo build --release --all'
else
	docker exec ${container} /root/.cargo/bin/cargo build --release --all
fi 


echo "Packaging"
if [[ "$distro" == centos* ]]
	then
	docker exec ${container} rpmbuild --define "_builddir $path" -bb gluster-collector.spec
	docker cp ${container}:/root/rpmbuild/RPMS/x86_64/* target/release/
	ls $path/target/release/
elif [[ "$distro" == ubuntu* ]]
	then
	echo "Installing cargo deb"
	docker exec ${container} /root/.cargo/bin/cargo install cargo-deb
	docker exec ${container} /root/.cargo/bin/cargo deb
	echo "cp /build/target/debian/*.deb target/release/"
	docker cp ${container}:/build/target/debian .
fi

# finish
