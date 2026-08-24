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
          borderRadius: 5,
          colorBgContainer: "#fffefa",
          colorBorder: "#d8d5cd",
          colorError: "#a63d35",
          colorPrimary: "#236b5b",
          colorText: "#1b1d1a",
          colorTextSecondary: "#6d716b",
          controlHeight: 34,
          fontFamily: "\"Avenir Next\", \"PingFang SC\", \"Hiragino Sans GB\", \"Microsoft YaHei\", sans-serif",
          fontSize: 13,
        },
        components: {
          Button: {
            defaultShadow: "none",
            fontWeight: 600,
            primaryShadow: "none",
          },
          Modal: {
            borderRadiusLG: 10,
          },
          Tabs: {
            inkBarColor: "#236b5b",
            itemActiveColor: "#184f43",
            itemSelectedColor: "#184f43",
          },
        },
      }}
    >
      <App />
    </ConfigProvider>
  </React.StrictMode>,
);
