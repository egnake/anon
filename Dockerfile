FROM kalilinux/kali-rolling

# Install required runtime and security dependencies
RUN apt-get update && apt-get install -y \
    make \
    tar \
    tor \
    curl \
    iproute2 \
    iptables \
    macchanger \
    obfs4proxy \
    snowflake-client \
    dnscrypt-proxy \
    proxychains4 \
    rfkill \
    secure-delete \
    i2pd \
    firejail \
    mat2 \
    libimage-exiftool-perl \
    libnotify-bin \
    psmisc \
    iputils-ping \
    ca-certificates \
    sudo \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Copy source files
WORKDIR /opt/anon
COPY . /opt/anon

# Install Anon globally
RUN make install

# Start the application
ENTRYPOINT ["anon"]
CMD ["--start"]
