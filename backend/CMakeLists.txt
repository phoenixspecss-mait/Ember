cmake_minimum_required(VERSION 3.16)
project(ember CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Threads REQUIRED)

add_executable(ember-server main.cpp analytics.cpp)

target_include_directories(ember-server PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_compile_options(ember-server PRIVATE -O2 -Wall -Wextra)

if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
  target_link_options(ember-server PRIVATE -static-libstdc++ -static-libgcc)
endif()

target_link_libraries(ember-server PRIVATE Threads::Threads)
