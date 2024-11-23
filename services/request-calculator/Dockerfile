# Use the official Rust image as the base
FROM rust:latest

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the current directory contents into the container
COPY . .

# Install dependencies and build the application
RUN cargo build --release

# Command to run the compiled binary
CMD ["./target/release/request-calculator"]
