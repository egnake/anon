FROM kalilinux/kali-rolling

# Install required dependencies
RUN apt-get update && apt-get install -y \
    make \
    tar \
    tor \
    curl \
    iproute2 \
    iptables \
    macchanger \
    obfs4proxy \
    proxychains4 \
    rfkill \
    secure-delete \
    i2pd \
    mat2 \
    libimage-exiftool-perl \
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
