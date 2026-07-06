import React from "react";
import { reportError } from "../services/errors/reportError";
import { toUserMessage } from "../services/errors/toUserMessage";
import i18n from "../../lang/i18n";
import { Button } from "../../components/ui/be-ui-button";

export class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { hasError: false, error: null, showDetails: false };
  }

  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }

  componentDidCatch(error, errorInfo) {
    reportError("frontend", "error_boundary", error, { errorInfo });
  }

  render() {
    if (!this.state.hasError) return this.props.children;

    const appErr = this.state.error;
    const message = toUserMessage(appErr);

    return (
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          padding: 24,
          background: "linear-gradient(135deg, #0f172a, #111827)",
          color: "#f8fafc",
        }}
      >
        <div
          style={{
            width: "min(680px, 92vw)",
            background: "rgba(15, 23, 42, 0.9)",
            border: "1px solid rgba(148, 163, 184, 0.2)",
            borderRadius: 16,
            padding: 24,
            boxShadow: "0 20px 60px rgba(15, 23, 42, 0.6)",
          }}
        >
          <h2 style={{ fontSize: 20, marginBottom: 8 }}>{message.title}</h2>
          <p style={{ opacity: 0.9, lineHeight: 1.5 }}>{message.message}</p>

          <div style={{ marginTop: 16, display: "flex", gap: 12 }}>
            <Button variant="outline" size="sm"
              onClick={() => this.setState((s) => ({ showDetails: !s.showDetails }))}
            >
              {this.state.showDetails ? i18n.t('errors.hideDetails') : i18n.t('errors.showDetails')}
            </Button>
            <Button variant="primary" size="sm"
              onClick={() => globalThis.location.reload()}
            >
              {i18n.t('errors.reload')}
            </Button>
          </div>

          {this.state.showDetails && (
            <pre
              style={{
                marginTop: 16,
                padding: 12,
                background: "rgba(15, 23, 42, 0.6)",
                borderRadius: 10,
                fontSize: 12,
                overflowX: "auto",
                maxHeight: 260,
              }}
            >
              {JSON.stringify(message.details, null, 2)}
            </pre>
          )}
        </div>
      </div>
    );
  }
}
