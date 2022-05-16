FROM ubuntu:latest

RUN apt-get update -y
RUN apt-get install -y git build-essential cmake libssl-dev curl gcc-arm-none-eabi binutils-arm-none-eabi libclang-dev clang pkg-config

SHELL ["/bin/bash", "-c"]
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add thumbv7em-none-eabihf
RUN rustup component add llvm-tools-preview
RUN cargo install cargo-binutils

# Python
RUN curl -L -O "https://github.com/conda-forge/miniforge/releases/latest/download/Mambaforge-$(uname)-$(uname -m).sh"
RUN bash "Mambaforge-$(uname)-$(uname -m).sh" -b
RUN rm "Mambaforge-$(uname)-$(uname -m).sh"
RUN /root/mambaforge/bin/mamba init

ADD . work/
WORKDIR work/
