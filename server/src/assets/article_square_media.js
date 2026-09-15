/* Browser-native bounded media preparation; no key or private URL is persisted. */
(() => {
  async function canvasUpload(source, width, height, request) {
    const c=document.createElement('canvas'), scale=Math.min(1,1600/Math.max(width,height)); c.width=Math.max(1,Math.round(width*scale)); c.height=Math.max(1,Math.round(height*scale));
    const ctx=c.getContext('2d'); if(!ctx) throw Error('无法处理媒体'); ctx.fillStyle='#fff';ctx.fillRect(0,0,c.width,c.height);ctx.drawImage(source,0,0,c.width,c.height);
    let data;for(const q of [.85,.7,.55,.4]) {data=c.toDataURL('image/jpeg',q);if(data.length<699000) break;}if(data.length>=699000) throw Error('图片过大，请裁剪');return request('/media','POST',{base64:data.split(',')[1]});
  }
  async function image(file,request) {
    if(!/^image\/(png|jpeg|webp)$/.test(file.type)||file.size>20*1024*1024) throw Error('请选择小于20MB的PNG、JPEG或WebP图片');
    const url=URL.createObjectURL(file);try {const img=new Image();img.src=url;await img.decode();return await canvasUpload(img,img.width,img.height,request);}finally{URL.revokeObjectURL(url);}
  }
  async function video(file,request) {
    if(!['video/mp4','video/webm'].includes(file.type)||file.size>32*1024*1024) throw Error('请选择小于32MB的MP4或WebM视频');
    const url=URL.createObjectURL(file),v=document.createElement('video');v.muted=true;v.playsInline=true;v.preload='auto';
    try {
      await new Promise((resolve,reject)=>{const t=setTimeout(()=>reject(Error('视频读取超时')),15000);v.onloadeddata=()=>{clearTimeout(t);resolve();};v.onerror=()=>{clearTimeout(t);reject(Error('视频无法读取'));};v.src=url;v.load();});
      if(!Number.isFinite(v.duration)||v.duration<=0||v.duration>600) throw Error('视频时长需在10分钟以内');
      const cover=await canvasUpload(v,v.videoWidth,v.videoHeight,request),form=new FormData();form.append('cover_id',cover.id);form.append('duration',String(v.duration));form.append('video',file);
      const result=await request('/videos','POST',form);return{id:result.id,name:file.name,cover:cover.data_url};
    }finally{v.pause();v.removeAttribute('src');v.load();URL.revokeObjectURL(url);}
  }
  window.ElonSquareMedia={image,video};
})();
