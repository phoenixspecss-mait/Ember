FROM debian:bookworm-slim AS build

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential cmake curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

COPY CMakeLists.txt main.cpp analytics.hpp analytics.cpp seed.hpp ./

RUN curl -fsSL https://raw.githubusercontent.com/yhirose/cpp-httplib/master/httplib.h -o httplib.h

RUN cmake -S . -B build -DCMAKE_BUILD_TYPE=Release \
    && cmake --build build --parallel

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 ember

COPY --from=build /src/build/ember-server /usr/local/bin/ember-server

USER ember

ENV PORT=10000

EXPOSE 10000

ENTRYPOINT ["/usr/local/bin/ember-server"]
