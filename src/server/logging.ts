import { resolveLogger, type LoggerAdapterLogger, type NormalizedLoggerAdapter } from "@package/logger-adapter";
import { PACKAGE_NAME } from "#5r46asnbaeq8";

function insightsLog(logger?: LoggerAdapterLogger | null): NormalizedLoggerAdapter {
  return resolveLogger({
      defaultLogger: false,
      fallback: "noop",
      logger: logger || undefined,
      source: PACKAGE_NAME,
  });
}

export { insightsLog };
