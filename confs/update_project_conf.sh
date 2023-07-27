#!/bin/bash
if [ $# -ne 1 ]; then
    echo "USAGE: $0 <release|debug>"
    echo " e.g.: $0 debug"
    exit 1
fi
MODE=$1

copy_conf_files() {

    cp $MODE/"h264.Cargo.toml" "../library/codec/h264/Cargo.toml"
    cp $MODE/"logger.Cargo.toml" "../library/logger/Cargo.toml"
    cp $MODE/"mpegts.Cargo.toml" "../library/container/mpegts/Cargo.toml"
    cp $MODE/"flv.Cargo.toml" "../library/container/flv/Cargo.toml"
    cp $MODE/"streamhub.Cargo.toml" "../library/streamhub/Cargo.toml"
    cp $MODE/"hls.Cargo.toml" "../protocol/hls/Cargo.toml"
    cp $MODE/"httpflv.Cargo.toml" "../protocol/httpflv/Cargo.toml"
    cp $MODE/"rtmp.Cargo.toml" "../protocol/rtmp/Cargo.toml"
    cp $MODE/"rtsp.Cargo.toml" "../protocol/rtsp/Cargo.toml"
    cp $MODE/"pprtmp.Cargo.toml" "../application/pprtmp/Cargo.toml"
    cp $MODE/"xiu.Cargo.toml" "../application/xiu/Cargo.toml"
}

# do some operations
if [ "$MODE" == "release" ]; then
    echo "执行发布任务..."
    # 添加发布任务的代码
else
    echo "执行调试任务..."

fi
