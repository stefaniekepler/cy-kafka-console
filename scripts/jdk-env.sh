# shellcheck shell=bash
# 自动定位本机 JDK（默认大版本 25）并导出 JAVA_HOME / PATH 到当前会话。
# 设计：仅当 PATH 中的 java 不是目标大版本时才探测，因此 CI（PATH 已是 JDK 25）
# 行为完全不变；本机默认为 JDK 8 时，会自动找到 sdkman / java_home 中的 JDK 25。
# 用法：在脚本中 `source "$SCRIPT_DIR/jdk-env.sh"`。可用 JDK_REQUIRED_MAJOR 覆盖目标大版本。

JDK_REQUIRED_MAJOR="${JDK_REQUIRED_MAJOR:-25}"

# 解析某个 java 可执行文件的大版本号（JDK 8 形如 1.8.0 取不到 8，但与 25 比较恒不等，安全）
_jdk_major() {
  "$1" -version 2>&1 | head -1 | sed -E 's/.*version "([0-9]+).*/\1/'
}

ensure_jdk() {
  # 1) PATH 中的 java 已是目标版本（典型 CI 场景）→ 不改动
  if command -v java >/dev/null 2>&1 && [ "$(_jdk_major java)" = "$JDK_REQUIRED_MAJOR" ]; then
    return 0
  fi

  local candidate=""

  # 2) 现有 JAVA_HOME 命中
  if [ -n "${JAVA_HOME:-}" ] && [ -x "$JAVA_HOME/bin/java" ] \
     && [ "$(_jdk_major "$JAVA_HOME/bin/java")" = "$JDK_REQUIRED_MAJOR" ]; then
    candidate="$JAVA_HOME"
  fi

  # 3) sdkman 安装目录（本机默认就是这种情况）
  local sdk_java="${SDKMAN_DIR:-$HOME/.sdkman}/candidates/java"
  if [ -z "$candidate" ] && [ -d "$sdk_java" ]; then
    local d
    for d in "$sdk_java/${JDK_REQUIRED_MAJOR}".* "$sdk_java/${JDK_REQUIRED_MAJOR}"; do
      if [ -x "$d/bin/jlink" ]; then candidate="$d"; break; fi
    done
  fi

  # 4) macOS 系统注册的 JDK
  if [ -z "$candidate" ] && [ -x /usr/libexec/java_home ]; then
    candidate="$(/usr/libexec/java_home -v "$JDK_REQUIRED_MAJOR" 2>/dev/null || true)"
  fi

  if [ -n "$candidate" ] && [ -x "$candidate/bin/java" ]; then
    export JAVA_HOME="$candidate"
    export PATH="$JAVA_HOME/bin:$PATH"
    echo "自动定位到 JDK ${JDK_REQUIRED_MAJOR}：$JAVA_HOME"
  fi
}

ensure_jdk
