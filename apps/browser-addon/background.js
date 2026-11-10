const menuItemId = "browsify-open-url";

// Service workers don't have access to window object, use chrome directly
const browser = chrome;

// Create context menu when service worker starts
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: menuItemId,
    title: "Open with Browsify",
    contexts: ["link"],
  });
});

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== menuItemId) {
    return;
  }

  console.log("Browsify Browser Extension: Opening link in Browsify", info.linkUrl);

  chrome.tabs.update(tab.id, { url: "browsify://open/" + btoa(info.linkUrl) });
});
