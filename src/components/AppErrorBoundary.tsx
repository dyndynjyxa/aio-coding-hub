import { Component, type ErrorInfo, type ReactNode } from "react";
import { reportRenderError } from "../services/frontendErrorReporter";

type Props = {
  children: ReactNode;
};

type State = {
  hasError: boolean;
};

export class AppErrorBoundary extends Component<Props, State> {
  state: State = {
    hasError: false,
  };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    reportRenderError(error, { componentStack: errorInfo.componentStack ?? undefined });
  }

  private handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex min-h-screen items-center justify-center bg-secondary p-6 text-foreground">
          <div className="max-w-md rounded-xl border border-border bg-white dark:bg-secondary p-5 shadow-sm">
            <div className="text-base font-semibold">页面渲染异常</div>
            <div className="mt-2 text-sm text-muted-foreground">
              已记录错误日志，请重启应用后重试。如果问题重复出现，请在"设置 →
              数据管理"打开数据目录并提供 logs 文件。
            </div>
            <button
              type="button"
              onClick={this.handleReload}
              className="mt-4 rounded-lg bg-card px-4 py-2 text-sm font-medium text-white transition hover:bg-secondary dark:bg-secondary dark:text-foreground dark:hover:bg-muted"
            >
              重新加载
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
