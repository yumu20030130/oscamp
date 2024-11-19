#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>

int main()
{
    int fd;
    int ret;
    char fname[] = "test_file";
    char content[] = "hello, arceos!";
    char buf[64];

    printf("FileOps ...\n");
    fd = creat(fname, 0600);
    if (fd < 0) {
        printf("Create file error!\n");
        exit(-1);
    }
    ret = write(fd, content, strlen(content)+1);
    if (ret < 0) {
        printf("Write file error!\n");
        exit(-1);
    }
    close(fd);

    fd = open(fname, O_RDONLY);
    if (fd < 0) {
        printf("Open file error!\n");
        exit(-1);
    }
    ret = read(fd, buf, sizeof(buf));
    if (ret <= 0) {
        printf("Read file error!\n");
        exit(-1);
    }
    buf[ret] = 0;
    printf("Read back content: [%d] %s\n", ret, buf);
    close(fd);
    printf("FileOps ok!\n");
    return 0;
}
