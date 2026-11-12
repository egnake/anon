FROM kalilinux/kali-rolling AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    cargo \
    rustc \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/anon
COPY Cargo.toml /opt/anon/
COPY src /opt/anon/src
RUN cargo build --release

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

WORKDIR /opt/anon
COPY --from=builder /opt/anon/target/release/anon /usr/bin/anon
COPY assets /usr/share/anon/assets
COPY assets/anon.desktop /usr/share/applications/anon.desktop
RUN chmod 755 /usr/bin/anon \
    && mkdir -p /usr/share/anon/backups /usr/share/kali-menu/applications \
    && rm -f /usr/share/anon/assets/sources/bridges.txt \
    && chmod 644 /usr/share/anon/assets/sources/anon.js \
       /usr/share/anon/assets/sources/bridges.default.txt \
    && cp -f /usr/share/anon/assets/anon.desktop /usr/share/kali-menu/applications/anon.desktop

# Start the application
ENTRYPOINT ["anon"]
CMD ["--start"]
