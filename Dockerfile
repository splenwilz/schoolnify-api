FROM messense/rust-musl-cross:x86_64-musl as builder
ENV SQLX_OFFLINE=true
# RUN cargo install cargo-chef
WORKDIR /school_management_system

# FROM chef AS planner
# # Copy source code from previous stage
COPY . .
# # Generate info for caching dependencies
# RUN cargo chef prepare --recipe-path recipe.json

# FROM chef AS builder
# COPY --from=planner /school_management_system/recipe.json recipe.json
# # Build & cache dependencies
# RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# # Copy source code from previous stage
# COPY . .
# # Build application
RUN cargo build --release --target x86_64-unknown-linux-musl

# Create a new stage with a minimal image
FROM scratch
COPY --from=builder /school_management_system/target/x86_64-unknown-linux-musl/release/school_management_system /school_management_system
ENTRYPOINT ["/school_management_system"]
EXPOSE 3000