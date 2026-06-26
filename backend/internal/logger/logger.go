package logger

import (
	"os"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// New creates a new zap.Logger based on the provided level and format.
// level: "debug", "info", "warn", "error"
// format: "json" or "console"
func New(level string, format string) (*zap.Logger, error) {
	var zapLevel zapcore.Level
	if err := zapLevel.UnmarshalText([]byte(level)); err != nil {
		zapLevel = zapcore.InfoLevel
	}

	encoderConfig := zap.NewProductionEncoderConfig()
	encoderConfig.TimeKey = "ts"
	encoderConfig.EncodeTime = zapcore.ISO8601TimeEncoder

	var encoder zapcore.Encoder
	switch format {
	case "console":
		encoder = zapcore.NewConsoleEncoder(encoderConfig)
	default:
		encoder = zapcore.NewJSONEncoder(encoderConfig)
	}

	core := zapcore.NewCore(
		encoder,
		zapcore.AddSync(os.Stdout),
		zapLevel,
	)

	logger := zap.New(core, zap.AddCaller(), zap.AddStacktrace(zapcore.ErrorLevel))
	return logger, nil
}
