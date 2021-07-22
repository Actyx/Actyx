FROM justmoon/wix
# The justmoon/wix image sets us up as `wix`; we switch back to root here for apt, etc.
USER root
RUN apt-get update && \
	apt-get install build-essential pkg-config libgsf-1-dev libssl-dev libcurl4-openssl-dev -y && \
	wget https://www.openssl.org/source/openssl-1.1.1g.tar.gz && \
	tar -zxf openssl-1.1.1g.tar.gz && cd openssl-1.1.1g && \
	./config && make && make install && \
	rm -f /usr/bin/openssl && ln -s /usr/local/bin/openssl /usr/bin/openssl && \
	ldconfig && \
	cd ../ && \
	wget https://github.com/mtrojnar/osslsigncode/releases/download/2.1/osslsigncode-2.1.0.tar.gz && \
	tar -xvf osslsigncode-2.1.0.tar.gz && cd osslsigncode-2.1.0/ && \
	./configure && make && make install
# Back to wix user
USER wix

