FROM ubuntu:latest

RUN apt-get update -y
RUN apt-get install -y git build-essential cmake libssl-dev curl gcc-arm-none-eabi binutils-arm-none-eabi

SHELL ["/bin/bash", "-c"]
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add thumbv7em-none-eabihf
RUN rustup component add llvm-tools-preview
RUN cargo install cargo-binutils

# Python
RUN curl -L -O "https://github.com/conda-forge/miniforge/releases/latest/download/Mambaforge-$(uname)-$(uname -m).sh" | sh -s -- -y

ADD . work/
WORKDIR work/
