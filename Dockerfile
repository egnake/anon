FROM kalilinux/kali-rolling

# Install required dependencies
RUN apt-get update && apt-get install -y \
    make \
    tar \
    tor \
    curl \
    python3 \
    python3-scapy \
    network-manager \
    secure-delete \
    iptables \
    sudo \
    macchanger \
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
