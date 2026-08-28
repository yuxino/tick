import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";
import "antd/dist/reset.css";
import App from "./App";
import "./App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ConfigProvider
      locale={zhCN}
      theme={{
        token: {
          borderRadius: 7,
          colorBgContainer: "#ffffff",
          colorBorder: "#d9d9d6",
          colorError: "#b2463d",
          colorPrimary: "#20201f",
          colorText: "#20201f",
          colorTextSecondary: "#6f6f6b",
          controlHeight: 32,
          fontFamily: "-apple-system, BlinkMacSystemFont, \"SF Pro Text\", \"PingFang SC\", \"Hiragino Sans GB\", sans-serif",
          fontSize: 13,
        },
        components: {
          Button: {
            defaultShadow: "none",
            fontWeight: 600,
            primaryShadow: "none",
          },
          Modal: {
            borderRadiusLG: 12,
          },
          Tabs: {
            inkBarColor: "#20201f",
            itemActiveColor: "#20201f",
            itemSelectedColor: "#20201f",
          },
        },
      }}
    >
      <App />
    </ConfigProvider>
  </React.StrictMode>,
);
