CC=g++
CFLAGS=-I./main -Wall -O3
TARGET=axicor_lite
SRCS=main/main.cpp

all: $(TARGET)

$(TARGET): $(SRCS) main/axicor_core.hpp
	$(CC) $(CFLAGS) $(SRCS) -o $(TARGET)

clean:
	rm -f $(TARGET)

run: all
	./$(TARGET)
