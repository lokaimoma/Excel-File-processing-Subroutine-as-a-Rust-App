"use strict";

const SERVER_URL = "http://localhost:6070";
const excelFileField = document.querySelector("#excelFile");
const excelFileName = document.querySelector("#fileName");
const contractionFile = document.querySelector("#contraFile");
const uploadStat = document.querySelector("#uploadStat");
const startJob = document.querySelector("#startJob");
const colHeaderCont = document.querySelector("#headerDisp");
const sortDateChkForm = document.querySelector("#sortDateCheckForm");

startJob.addEventListener("click", function (_) {
  startJob.disabled = true;
  startJob.textContent = "Please wait";
  const excelFileForm = document.querySelector("#excelFileForm");
  const form = new FormData(excelFileForm);
  fetch(`${SERVER_URL}/upload`, {
    method: "post",
    body: form,
  })
    .then((response) => {
      console.log(response);
      if (!response.ok) {
        response.text().then((txt) => {
          console.error(txt);
        });
      } else {
        console.log("File uploaded successfully");
        return response.json();
      }
    })
    .catch((e) => {
      console.log("Error connecting: ", e);
    })
    .then((json) => {
      if (!json) return;
      const excelFileField = json["id"];
      getHeaderRow(excelFileField).then((json) => {
        const headerPopulatorJob = new Promise((resolve, _) => {
          const documentFragment = document.createDocumentFragment();
          for (const columnTitle of json["columns"]) {
            const p = document.createElement("p");
            p.textContent = columnTitle;
            documentFragment.appendChild(p);
          }
          colHeaderCont.appendChild(documentFragment);
          resolve();
        });

        const sortCtrlsPoulatorJob = new Promise((resolve, _) => {
          const sortCtrlsFragment = document.createDocumentFragment();
          for (let i = 1; i <= json["columns"].length; i++) {
            const sortGroup = document.createElement("div");
            sortGroup.className = "sortGroup";

            const flexLayer1 = createSortControl(i, "A-Z Sort", "asc", "desc");
            const flexLayer2 = createSortControl(i, "Z-A Sort", "desc", "asc");
            const flexLayer3 = createCheckDateCtrl(i);

            sortGroup.appendChild(flexLayer1);
            sortGroup.appendChild(flexLayer2);
            sortGroup.appendChild(flexLayer3);
            sortCtrlsFragment.appendChild(sortGroup);
          }
          sortDateChkForm.appendChild(sortCtrlsFragment);
        });

        return Promise.all([headerPopulatorJob, sortCtrlsPoulatorJob]);
      });
    })
    .then((_, __) => {})
    .finally(() => {
      startJob.disabled = false;
      startJob.textContent = "Start Job";
    });
});

/**
 *
 * @param {number} columnNumber The column number
 * @returns {HTMLElement}
 */
function createCheckDateCtrl(columnNumber) {
  const flex = document.createElement("div");
  flex.className = "flex gap-s";
  const inputBgRed = document.createElement("div");
  inputBgRed.className = "inputBgBlue";

  const chkInput = document.createElement("input");
  chkInput.setAttribute("type", "checkbox");
  chkInput.setAttribute("name", "checkDate");
  chkInput.setAttribute("id", `checkDate-col-${columnNumber}`);
  chkInput.setAttribute("value", `${columnNumber}`);
  const label = document.createElement("label");
  label.setAttribute("for", `checkDate-col-${columnNumber}`);
  label.textContent = "mmddyy Checking";

  inputBgRed.appendChild(chkInput);
  flex.appendChild(inputBgRed);
  flex.appendChild(label);
  return flex;
}

/**
 *
 * @param {number} columnNumber The column number
 * @param {String} labelTxt The label text `A-Z Sort` or `Z-A Sort`
 * @param {String} order The order of sort `asc` or `desc`
 * @param {String} opp Opposite of the order value `desc` for `asc` and vice-versa
 * @returns {HTMLElement}
 */
function createSortControl(columnNumber, labelTxt, order, opp) {
  const flexLayer = document.createElement("div");
  flexLayer.className = "flex gap-s";
  const inputBgRed = document.createElement("div");
  inputBgRed.className = "inputBgRed";

  const chkBxInpt = document.createElement("input");
  chkBxInpt.setAttribute("type", "checkbox");
  chkBxInpt.dataset.rowId = columnNumber;
  chkBxInpt.dataset.opp = `col-${columnNumber}-${opp}`;
  chkBxInpt.setAttribute("name", "sortCol");
  chkBxInpt.setAttribute("id", `col-${columnNumber}-${order}`);
  chkBxInpt.setAttribute("value", `${order},${order}`);
  const chkLabel = document.createElement("label");
  chkLabel.setAttribute("for", `col-${columnNumber}-${order}`);
  chkLabel.textContent = labelTxt;

  inputBgRed.appendChild(chkBxInpt);
  flexLayer.appendChild(inputBgRed);
  flexLayer.appendChild(chkLabel);
  return flexLayer;
}

/**
 *
 * @param {String} id
 * @returns {Promise<String[]>}
 */
function getHeaderRow(id) {
  return fetch(`${SERVER_URL}/getHeader/${id}`).then((response) => {
    if (!response.ok) {
      response.text().then((txt) => console.error(txt));
    } else {
      console.log("Header request successful");
      return response.json();
    }
  });
}

excelFileField.addEventListener("change", function (e) {
  const target = e.target;
  if (target.files.length == 0) {
    excelFileName.textContent = "No file selected";
    startJob.disabled = true;
  } else {
    excelFileName.textContent = target.files[0].name;
    startJob.disabled = false;
  }
});

contractionFile.addEventListener("change", function (e) {
  const target = e.target;
  if (target.files.length == 0) {
    uploadStat.classList.remove("uploaded");
  } else {
    uploadStat.classList.add("uploaded");
  }
});

/**
 *
 * @param {Event} event
 */
function uncheck_counter(event) {
  /**
   * @param {HTMLElement}
   */
  const target = event.target;
  if (!target.checked) {
    return;
  }

  /**
   * @type(string | undefined)
   */
  const opp_element = target.dataset.opp;
  if (opp_element) {
    /**
     * @type {HtmlElement | Undefined}
     */
    const oppelement = document.querySelector(`#${opp_element}`);

    if (oppelement) {
      oppelement.checked = false;
    }
  }
}
