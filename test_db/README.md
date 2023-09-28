How to recreate this build

From within this directory:

```shell
export IMAGE_NAME=bh2smith/test-events
docker build -t $IMAGE_NAME .
docker push $IMAGE_NAME

docker run $IMAGE_NAME
```
