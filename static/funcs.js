var _open = true;
function toggledetail() {
  var ds = document.getElementsByTagName("details");
  var len = ds.length;
  for (var i = 0; i < len; i++) {
    if (_open) {
      ds[i].removeAttribute("open");
    } else {
      ds[i].setAttribute("open", "");
    }
  }
  _open = !_open;
}
